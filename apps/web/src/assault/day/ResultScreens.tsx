// The transition and terminal screens after an Assault fight. They receive the record and verbs
// they render; the Assault reducer already decided the result.

import type { ReactElement } from "react";

import type { AssaultRecord } from "@jackioh/run";

import { t } from "../../i18n/i18n.ts";
import { paths } from "../../net/navigate.ts";
import { BackLink } from "../../routes/nav.tsx";
import { Wordmark } from "../../routes/Wordmark.tsx";
import { assaultTestid } from "../testids.ts";

type ResultFrameProps = { children: ReactElement | ReactElement[] };

function ResultFrame({ children }: ResultFrameProps): ReactElement {
  return (
    <div className="app-shell tavern assault-screen" data-testid={assaultTestid.result}>
      <BackLink to={paths.play} title={t("Infinite Assault")} />
      <section className="panel panel--auth assault-panel">
        <Wordmark />
        {children}
      </section>
    </div>
  );
}

function RecordLine({ record }: { record: AssaultRecord }): ReactElement {
  return (
    <p data-testid={assaultTestid.record}>
      {t("Best: level {best} · Attempts: {attempts}", {
        best: String(record.best),
        attempts: String(record.attempts),
      })}
    </p>
  );
}

export function AfterLevelScreen({ beaten, next, onContinue }: { beaten: number; next: number; onContinue(): void }): ReactElement {
  return (
    <ResultFrame>
      <h2>{t("Level {level} beaten", { level: String(beaten) })}</h2>
      <p>{t("The next enemy is waiting at level {level}.", { level: String(next) })}</p>
      <button type="button" className="button-primary" onClick={onContinue}>{t("Continue")}</button>
    </ResultFrame>
  );
}

export function LostScreen({ record, onTryAgain, onBack }: { record: AssaultRecord; onTryAgain(): void; onBack(): void }): ReactElement {
  return (
    <ResultFrame>
      <h2>{t("The Assault repelled you")}</h2>
      <p>{t("Your record is kept. Start again from level 1 or your highest checkpoint.")}</p>
      <RecordLine record={record} />
      <div className="assault-actions">
        <button type="button" className="button-primary" onClick={onTryAgain}>{t("Try again")}</button>
        <button type="button" onClick={onBack}>{t("Back")}</button>
      </div>
    </ResultFrame>
  );
}

export function RepelledScreen({ record, turns, onBack }: { record: AssaultRecord; turns: number; onBack(): void }): ReactElement {
  return (
    <ResultFrame>
      <h2>{t("Assault repelled")}</h2>
      <p>{t("You cleared today's Infinite Assault in {turns} turns.", { turns: String(turns) })}</p>
      <RecordLine record={record} />
      <button type="button" className="button-primary" onClick={onBack}>{t("Back to today")}</button>
    </ResultFrame>
  );
}

export function AbandonedScreen({ record, onBack }: { record: AssaultRecord; onBack(): void }): ReactElement {
  return (
    <ResultFrame>
      <h2>{t("Attempt abandoned")}</h2>
      <p>{t("The day's record is kept.")}</p>
      <RecordLine record={record} />
      <button type="button" className="button-primary" onClick={onBack}>{t("Back to today")}</button>
    </ResultFrame>
  );
}
