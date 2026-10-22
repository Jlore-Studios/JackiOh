// `/creator`: the card creator (#920, SPEC §12.9), behind the gate like the deck builder, since a
// custom card is kept on the account. The page is `creator/Creator.tsx`; this route gives it the
// session's token and sends "Try it" to a practice game with the saved card (R915009).
// Back leads to the Collection hub (R1579000). While the editor holds changes not yet saved, Back shows the creator's unsaved-changes note and a reload or a closed tab makes the browser ask first (R1484000).

import { useCallback, useEffect, useState, type ReactElement } from "react";

import { Creator, UnsavedNote, type CreatorApi } from "../creator/Creator.tsx";
import { t } from "../i18n/i18n.ts";
import { navigate, paths, tryPath } from "../net/navigate.ts";
import { BackLink } from "./nav.tsx";

import "../auth/tavern.css";

export type CreatorRouteProps = {
  token: string;
  /** default `DEFAULT_CREATOR_API` */
  api?: CreatorApi;
};

export default function CreatorRoute({ token, api }: CreatorRouteProps): ReactElement {
  /** Whether the editor holds changes not yet saved, as `Creator` reports it. */
  const [unsaved, setUnsaved] = useState(false);
  /** Back was pressed with unsaved changes: the note asks whether to discard them. */
  const [leaving, setLeaving] = useState(false);
  const onUnsaved = useCallback((next: boolean) => {
    setUnsaved(next);
    if (!next) setLeaving(false);
  }, []);

  // A draft lives only in the page, so while it holds unsaved changes a reload or a closed tab
  // asks first, as a lesson in progress does (R668, R1484000).
  useEffect(() => {
    if (!unsaved) return;
    const onBeforeUnload = (event: BeforeUnloadEvent): void => {
      event.preventDefault();
      // Older engines show the prompt only when returnValue is set.
      event.returnValue = "";
    };
    window.addEventListener("beforeunload", onBeforeUnload);
    return () => {
      window.removeEventListener("beforeunload", onBeforeUnload);
    };
  }, [unsaved]);

  return (
    <div className="app-shell app-shell--wide tavern creator-screen">
      <BackLink
        onPress={() => {
          if (unsaved) setLeaving(true);
          else navigate(paths.collection);
        }}
      />
      <h1 className="creator-title">{t("Card creator")}</h1>
      <p className="creator-hint">{t("Make your own cards and try them against the AI. Custom cards play only in practice.")}</p>
      {leaving ? <UnsavedNote onDiscard={() => navigate(paths.collection)} onKeep={() => setLeaving(false)} /> : null}
      <Creator token={token} {...(api === undefined ? {} : { api })} onTry={(id) => navigate(tryPath(id))} onUnsaved={onUnsaved} />
    </div>
  );
}
