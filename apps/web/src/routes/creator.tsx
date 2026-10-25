// `/creator`: the card creator (#920, SPEC §12.9), behind the gate like the deck builder, since a
// custom card is kept on the account. The page is `creator/Creator.tsx`; this route gives it the
// session's token and sends "Try it" to a practice game with the saved card (R915009).
// Back leads to the Collection hub (R1579000). While the editor holds changes not yet saved, Back shows the creator's unsaved-changes note and a reload or a closed tab makes the browser ask first (R1484000).
// Every other way off it within the client, a pairing's or a challenge's move and the browser's Back among them, shows the same note (R1754000).

import { useCallback, useEffect, useState, type ReactElement } from "react";

import { Creator, UnsavedNote, type CreatorApi } from "../creator/Creator.tsx";
import { t } from "../i18n/i18n.ts";
import { guardLeaving, navigate, paths, tryPath } from "../net/navigate.ts";
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
  /** A way off the page taken with unsaved changes: the note asks whether to discard them, then makes that move (R1484000, R1754000). */
  const [leaving, setLeaving] = useState<{ leave: () => void } | null>(null);
  const onUnsaved = useCallback((next: boolean) => {
    setUnsaved(next);
    if (!next) setLeaving(null);
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
    // Every other way off the page waits for the note (R1754000).
    const release = guardLeaving((leave) => {
      setLeaving({ leave });
    });
    return () => {
      window.removeEventListener("beforeunload", onBeforeUnload);
      release();
    };
  }, [unsaved]);

  return (
    <div className="app-shell app-shell--wide tavern creator-screen one-screen">
      <BackLink
        onPress={() => {
          navigate(paths.collection);
        }}
      />
      <h1 className="creator-title">{t("Card creator")}</h1>
      <p className="creator-hint">{t("Make your own cards and try them against the AI. Custom cards play only in practice.")}</p>
      {leaving === null ? null : (
        <UnsavedNote
          onDiscard={leaving.leave}
          onKeep={() => {
            setLeaving(null);
          }}
        />
      )}
      <Creator
        token={token}
        {...(api === undefined ? {} : { api })}
        onTry={(id) => {
          // Try it saves first, but `unsaved` turns false here only a render later (R1754000).
          navigate(tryPath(id), { force: true });
        }}
        onUnsaved={onUnsaved}
      />
    </div>
  );
}
