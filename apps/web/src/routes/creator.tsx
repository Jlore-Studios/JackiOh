// `/creator`: the card creator (#920, SPEC §12.9), behind the gate like the deck builder, since a
// custom card is kept on the account. The page is `creator/Creator.tsx`; this route gives it the
// session's token and sends "Try it" to a practice game with the saved card (R915009).

import type { ReactElement } from "react";

import { Creator, type CreatorApi } from "../creator/Creator.tsx";
import { t } from "../i18n/i18n.ts";
import { navigate, tryPath } from "../net/navigate.ts";
import { BackLink } from "./nav.tsx";

import "../auth/tavern.css";

export type CreatorRouteProps = {
  token: string;
  /** default `DEFAULT_CREATOR_API` */
  api?: CreatorApi;
};

export default function CreatorRoute({ token, api }: CreatorRouteProps): ReactElement {
  return (
    <div className="app-shell app-shell--wide tavern creator-screen">
      <BackLink />
      <h1 className="creator-title">{t("Card creator")}</h1>
      <p className="creator-hint">{t("Make your own cards and try them against the AI. Custom cards play only in practice.")}</p>
      <Creator token={token} {...(api === undefined ? {} : { api })} onTry={(id) => navigate(tryPath(id))} />
    </div>
  );
}
