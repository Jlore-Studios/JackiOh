// `/creator`: the card creator (#920, SPEC §12.9), behind the gate since a custom card is kept on the
// account; this route gives `creator/Creator.tsx` the token and sends "Try it" to a practice game
// with the saved card (R915009). Back leads to the Collection hub (R1579000).

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
  const [unsaved, setUnsaved] = useState(false);
  /** A way off taken with unsaved changes, held until the note answers (R1484000, R1754000). */
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
        to={paths.collection}
        onPress={() => {
          navigate(paths.collection);
        }}
      />
      <h1 className="creator-title">{t("Card creator")}</h1>
      <p className="creator-hint">{t("Custom cards play only in practice.")}</p>
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
